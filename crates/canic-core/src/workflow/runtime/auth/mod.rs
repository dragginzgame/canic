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

///
/// RuntimeAuthWorkflow
///
/// Owns delegated-auth runtime startup checks and auth-specific runtime boot
/// logging for root and non-root canisters.
///

pub struct RuntimeAuthWorkflow;

impl RuntimeAuthWorkflow {
    /// Fail fast when root delegated-auth config requires missing crypto support.
    pub fn ensure_root_crypto_contract() -> Result<(), InternalError> {
        let cfg = ConfigOps::get()?;
        if root_requires_delegated_token_proofs(&cfg)
            && !AuthOps::root_canister_sig_create_enabled()
        {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "delegated token signing is configured in canic.toml, but this root build does not include IC canister-signature creation support; enable the `auth-root-canister-sig-create` feature for the root canister build".to_string(),
            ));
        }

        if root_requires_role_attestation_public_keys(&cfg)
            && !EcdsaOps::threshold_public_key_fetch_enabled()
        {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "root auth public-key certification is configured in canic.toml, but this root build does not include threshold ECDSA public-key fetch support; enable the `auth-threshold-ecdsa-public-key` feature for the root canister build".to_string(),
            ));
        }

        Ok(())
    }

    /// Fail fast when one delegated signer canister lacks threshold ECDSA support.
    pub fn ensure_nonroot_crypto_contract(
        canister_role: &CanisterRole,
        canister_cfg: &crate::config::schema::CanisterConfig,
    ) -> Result<(), InternalError> {
        if nonroot_requires_delegated_token_issuer(canister_cfg)
            && !AuthOps::issuer_canister_sig_create_enabled()
        {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!(
                    "canister '{canister_role}' is configured as a delegated auth issuer, but this build does not include IC canister-signature creation support; enable the `auth-issuer-canister-sig-create` feature for that canister build",
                ),
            ));
        }

        Self::ensure_delegated_token_verifier_contract(canister_role, canister_cfg)?;

        Ok(())
    }

    /// Fail fast when a non-root delegated-token verifier lacks hard-cut trust anchors.
    fn ensure_delegated_token_verifier_contract(
        canister_role: &CanisterRole,
        canister_cfg: &crate::config::schema::CanisterConfig,
    ) -> Result<(), InternalError> {
        let delegated_tokens_cfg = ConfigOps::delegated_tokens_config()?;
        if !delegated_tokens_cfg.enabled || !nonroot_requires_delegated_token_verifier(canister_cfg)
        {
            return Ok(());
        }

        if !AuthOps::root_canister_sig_verify_enabled() {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!(
                    "canister '{canister_role}' has delegated token verification enabled, but this build does not include IC canister-signature verification support; enable the `auth-delegated-token-verify` or `auth-root-canister-sig-verify` feature",
                ),
            ));
        }

        AuthOps::delegated_token_verifier_config().map(|_| ())
    }

    /// Check local issuer support when the current canister mints delegated tokens.
    pub async fn check_signer_key_material() -> Result<(), InternalError> {
        std::future::ready(()).await;
        let delegated_tokens_cfg = ConfigOps::delegated_tokens_config()?;
        let canister_cfg = ConfigOps::current_canister()?;
        if !delegated_tokens_cfg.enabled || !canister_cfg.auth.delegated_token_signer {
            return Ok(());
        }

        crate::log!(
            Topic::Auth,
            Info,
            "delegated-token issuer canister-signature support checked issuer={}",
            IcOps::canister_self()
        );

        Ok(())
    }

    /// Ensure legacy delegated-grant root trust material is published in subnet state.
    pub async fn publish_root_delegated_grant_key_to_subnet_state() -> Result<(), InternalError> {
        EnvOps::require_root()?;

        let delegated_tokens_cfg = ConfigOps::delegated_tokens_config()?;
        if !delegated_tokens_cfg.enabled {
            return Ok(());
        }
        if !EcdsaOps::threshold_public_key_fetch_enabled() {
            log!(
                Topic::Auth,
                Debug,
                "skipping legacy delegated-grant root public-key publication because threshold ECDSA public-key fetch support is not compiled in"
            );
            return Ok(());
        }

        AuthOps::publish_delegated_grant_root_key_material().await
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
        let now_ns = IcOps::now_nanos();
        let verifier_subnet = Some(EnvOps::subnet_pid()?);
        let root_pid = EnvOps::root_pid()?;

        let verify = || {
            AuthOps::verify_role_attestation_cached(
                attestation,
                caller,
                self_pid,
                verifier_subnet,
                now_ns,
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
        attestation.payload.issued_at_ns,
        attestation.payload.expires_at_ns,
        attestation.payload.epoch,
        err
    );
}

// Decide whether the root runtime must create canister-signature root proofs.
fn root_requires_delegated_token_proofs(cfg: &ConfigModel) -> bool {
    cfg.subnets.values().any(|subnet| {
        subnet.canisters.values().any(|canister| {
            cfg.auth.delegated_tokens.enabled && canister.auth.delegated_token_signer
        })
    })
}

fn root_requires_role_attestation_public_keys(cfg: &ConfigModel) -> bool {
    cfg.subnets.values().any(|subnet| {
        subnet
            .canisters
            .values()
            .any(|canister| canister.auth.role_attestation_cache)
    })
}

// Decide whether one non-root runtime must create issuer canister signatures.
const fn nonroot_requires_delegated_token_issuer(
    canister_cfg: &crate::config::schema::CanisterConfig,
) -> bool {
    canister_cfg.auth.delegated_token_signer
}

// Decide whether one non-root runtime must carry delegated-token verifier support.
const fn nonroot_requires_delegated_token_verifier(
    canister_cfg: &crate::config::schema::CanisterConfig,
) -> bool {
    canister_cfg.auth.delegated_token_signer || canister_cfg.auth.role_attestation_cache
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeAuthWorkflow, nonroot_requires_delegated_token_issuer,
        nonroot_requires_delegated_token_verifier, root_requires_delegated_token_proofs,
        root_requires_role_attestation_public_keys,
    };
    use crate::{
        config::schema::{CanisterAuthConfig, CanisterKind},
        ids::CanisterRole,
        test::config::ConfigTestBuilder,
    };

    #[test]
    fn root_requires_canister_signature_proofs_for_delegated_signer_when_enabled() {
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

        assert!(root_requires_delegated_token_proofs(&cfg));
        assert!(!root_requires_role_attestation_public_keys(&cfg));
    }

    #[test]
    fn root_requires_public_key_fetch_for_role_attestation_cache_when_delegated_tokens_disabled() {
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

        assert!(!root_requires_delegated_token_proofs(&cfg));
        assert!(root_requires_role_attestation_public_keys(&cfg));
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

        assert!(!root_requires_delegated_token_proofs(&cfg));
        assert!(!root_requires_role_attestation_public_keys(&cfg));
    }

    #[test]
    fn root_does_not_require_auth_crypto_without_auth_roles() {
        let cfg = ConfigTestBuilder::new().build();

        assert!(!root_requires_delegated_token_proofs(&cfg));
        assert!(!root_requires_role_attestation_public_keys(&cfg));
    }

    #[test]
    fn verifier_only_nonroot_does_not_require_auth_crypto() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_signer: false,
            role_attestation_cache: true,
        };

        assert!(!nonroot_requires_delegated_token_issuer(&verifier_cfg));
    }

    #[test]
    fn default_nonroot_does_not_require_delegated_token_verifier() {
        let cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);

        assert!(!nonroot_requires_delegated_token_verifier(&cfg));
    }

    #[test]
    fn auth_material_nonroot_requires_delegated_token_verifier() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_signer: false,
            role_attestation_cache: true,
        };

        let mut signer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        signer_cfg.auth = CanisterAuthConfig {
            delegated_token_signer: true,
            role_attestation_cache: false,
        };

        assert!(nonroot_requires_delegated_token_verifier(&verifier_cfg));
        assert!(nonroot_requires_delegated_token_verifier(&signer_cfg));
    }

    #[cfg(not(feature = "auth-root-canister-sig-verify"))]
    #[test]
    fn delegated_token_verifier_startup_requires_canister_signature_verify_feature() {
        let _ = ConfigTestBuilder::new().install();
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_signer: false,
            role_attestation_cache: true,
        };
        let role = CanisterRole::new("app");

        let err = RuntimeAuthWorkflow::ensure_nonroot_crypto_contract(&role, &verifier_cfg)
            .expect_err("expected verifier feature error");

        assert!(
            err.to_string().contains("auth-delegated-token-verify"),
            "expected delegated-token verifier feature error, got: {err}"
        );
    }

    #[test]
    fn signer_nonroot_requires_issuer_canister_signature_create() {
        let mut signer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        signer_cfg.auth = CanisterAuthConfig {
            delegated_token_signer: true,
            role_attestation_cache: true,
        };

        assert!(nonroot_requires_delegated_token_issuer(&signer_cfg));
    }

    #[test]
    fn runtime_auth_workflow_type_exists_for_runtime_ownership() {
        let _ = RuntimeAuthWorkflow;
    }
}
