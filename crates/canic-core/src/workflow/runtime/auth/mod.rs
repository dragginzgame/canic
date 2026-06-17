//! Module: workflow::runtime::auth
//!
//! Responsibility: orchestrate runtime auth startup checks and local verification.
//! Does not own: endpoint authorization, auth storage records, or crypto primitives.
//! Boundary: lifecycle and API layers call this after config/runtime context is available.

mod prepare;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::ConfigModel,
    dto::auth::SignedRoleAttestation,
    format::display_optional,
    ids::CanisterRole,
    ops::{
        auth::{AuthExpiryError, AuthOps, AuthOpsError},
        config::ConfigOps,
        ic::IcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            record_attestation_epoch_rejected, record_attestation_verify_failed,
        },
    },
    workflow::prelude::*,
};

///
/// RuntimeAuthWorkflow
///
/// Owns delegated-auth runtime startup checks and auth-specific runtime boot
/// logging for root and non-root canisters.
/// Owned by runtime workflow and consumed by lifecycle/API auth surfaces.
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
                "delegated token proof issuance is configured in canic.toml, but this root build does not include IC canister-signature creation support; enable the `auth-root-canister-sig-create` feature for the root canister build".to_string(),
            ));
        }

        if root_requires_role_attestation_proofs(&cfg)
            && !AuthOps::root_canister_sig_create_enabled()
        {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "role attestation issuance is configured in canic.toml, but this root build does not include IC canister-signature creation support; enable the `auth-root-canister-sig-create` feature for the root canister build".to_string(),
            ));
        }

        Ok(())
    }

    /// Fail fast when one delegated-token issuer lacks canister-signature support.
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

        Self::ensure_auth_proof_verifier_support_contract(canister_role, canister_cfg)?;

        Ok(())
    }

    /// Fail fast when a non-root auth verifier lacks hard-cut trust anchors.
    fn ensure_auth_proof_verifier_support_contract(
        canister_role: &CanisterRole,
        canister_cfg: &crate::config::schema::CanisterConfig,
    ) -> Result<(), InternalError> {
        let delegated_tokens_cfg = ConfigOps::delegated_tokens_config()?;
        if !nonroot_requires_root_proof_verifier_support(canister_cfg) {
            return Ok(());
        }

        if !AuthOps::root_canister_sig_verify_enabled() {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!(
                    "canister '{canister_role}' has auth proof verification enabled, but this build does not include root IC canister-signature verification support; enable the `auth-delegated-token-verify` or `auth-root-canister-sig-verify` feature",
                ),
            ));
        }

        if nonroot_requires_issuer_proof_verifier_support(canister_cfg)
            && !AuthOps::issuer_canister_sig_verify_enabled()
        {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!(
                    "canister '{canister_role}' has delegated-token verification enabled, but this build does not include issuer IC canister-signature verification support; enable the `auth-delegated-token-verify` or `auth-issuer-canister-sig-verify` feature",
                ),
            ));
        }

        if delegated_tokens_cfg.enabled || canister_cfg.auth.role_attestation_cache {
            AuthOps::auth_proof_verifier_config().map(|_| ())
        } else {
            Ok(())
        }
    }

    /// Check local canister-signature support when the current canister issues delegated tokens.
    pub async fn check_issuer_canister_signature_support() -> Result<(), InternalError> {
        // Keep the public runtime hook async without adding hot-path outbound work.
        std::future::ready(()).await;
        let delegated_tokens_cfg = ConfigOps::delegated_tokens_config()?;
        let canister_cfg = ConfigOps::current_canister()?;
        if !delegated_tokens_cfg.enabled || !canister_cfg.auth.delegated_token_issuer {
            return Ok(());
        }

        crate::log!(
            Topic::Auth,
            Info,
            "delegated-token issuer canister-signature support ready issuer={}",
            IcOps::canister_self()
        );

        Ok(())
    }

    /// Verify a role attestation locally from its embedded root proof.
    pub async fn verify_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), InternalError> {
        // This verifier is intentionally local. The await preserves the async
        // endpoint shape; do not add root, issuer, or management-canister calls here.
        std::future::ready(()).await;
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

        match AuthOps::verify_role_attestation_cached(
            attestation,
            caller,
            self_pid,
            verifier_subnet,
            now_ns,
            min_accepted_epoch,
        ) {
            Ok(_) => Ok(()),
            Err(err) => {
                record_attestation_verifier_rejection(&err);
                log_attestation_verifier_rejection(&err, attestation, caller, self_pid);
                Err(err.into())
            }
        }
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
    if let AuthOpsError::Expiry(AuthExpiryError::AttestationEpochRejected { .. }) = err {
        record_attestation_epoch_rejected();
    }
}

fn log_attestation_verifier_rejection(
    err: &AuthOpsError,
    attestation: &SignedRoleAttestation,
    caller: Principal,
    self_pid: Principal,
) {
    log!(
        Topic::Auth,
        Warn,
        "role attestation rejected local={} caller={} subject={} role={} audience={} subnet={} issued_at={} expires_at={} epoch={} error={}",
        self_pid,
        caller,
        attestation.payload.subject,
        attestation.payload.role,
        attestation.payload.audience,
        display_optional(attestation.payload.subnet_id),
        attestation.payload.issued_at_ns,
        attestation.payload.expires_at_ns,
        attestation.payload.epoch,
        err
    );
}

fn root_requires_delegated_token_proofs(cfg: &ConfigModel) -> bool {
    cfg.subnets.values().any(|subnet| {
        subnet.canisters.values().any(|canister| {
            cfg.auth.delegated_tokens.enabled && canister.auth.delegated_token_issuer
        })
    })
}

fn root_requires_role_attestation_proofs(cfg: &ConfigModel) -> bool {
    cfg.subnets.values().any(|subnet| {
        subnet
            .canisters
            .values()
            .any(|canister| canister.auth.role_attestation_cache)
    })
}

const fn nonroot_requires_delegated_token_issuer(
    canister_cfg: &crate::config::schema::CanisterConfig,
) -> bool {
    canister_cfg.auth.delegated_token_issuer
}

const fn nonroot_requires_root_proof_verifier_support(
    canister_cfg: &crate::config::schema::CanisterConfig,
) -> bool {
    canister_cfg.auth.delegated_token_issuer
        || canister_cfg.auth.delegated_token_verifier
        || canister_cfg.auth.role_attestation_cache
}

const fn nonroot_requires_issuer_proof_verifier_support(
    canister_cfg: &crate::config::schema::CanisterConfig,
) -> bool {
    canister_cfg.auth.delegated_token_verifier
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeAuthWorkflow, nonroot_requires_delegated_token_issuer,
        nonroot_requires_issuer_proof_verifier_support,
        nonroot_requires_root_proof_verifier_support, root_requires_delegated_token_proofs,
        root_requires_role_attestation_proofs,
    };
    use crate::{
        config::schema::{CanisterAuthConfig, CanisterKind},
        ids::CanisterRole,
        test::config::ConfigTestBuilder,
    };

    #[test]
    fn root_requires_canister_signature_proofs_for_delegated_issuer_when_enabled() {
        let mut issuer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        issuer_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: true,
            delegated_token_verifier: false,
            role_attestation_cache: false,
        };

        let cfg = ConfigTestBuilder::new()
            .with_prime_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_prime_canister("user_shard", issuer_cfg)
            .build();

        assert!(root_requires_delegated_token_proofs(&cfg));
        assert!(!root_requires_role_attestation_proofs(&cfg));
    }

    #[test]
    fn root_requires_canister_signature_proofs_for_role_attestation_cache_when_delegated_tokens_disabled()
     {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: false,
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
        assert!(root_requires_role_attestation_proofs(&cfg));
    }

    #[test]
    fn root_ignores_delegated_issuer_when_delegated_tokens_disabled() {
        let mut issuer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        issuer_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: true,
            delegated_token_verifier: false,
            role_attestation_cache: false,
        };

        let mut cfg = ConfigTestBuilder::new()
            .with_prime_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_prime_canister("user_shard", issuer_cfg)
            .build();
        cfg.auth.delegated_tokens.enabled = false;

        assert!(!root_requires_delegated_token_proofs(&cfg));
        assert!(!root_requires_role_attestation_proofs(&cfg));
    }

    #[test]
    fn root_does_not_require_auth_crypto_without_auth_roles() {
        let cfg = ConfigTestBuilder::new().build();

        assert!(!root_requires_delegated_token_proofs(&cfg));
        assert!(!root_requires_role_attestation_proofs(&cfg));
    }

    #[test]
    fn verifier_only_nonroot_requires_root_and_issuer_proof_verifier_support() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: true,
            role_attestation_cache: false,
        };

        assert!(!nonroot_requires_delegated_token_issuer(&verifier_cfg));
        assert!(nonroot_requires_root_proof_verifier_support(&verifier_cfg));
        assert!(nonroot_requires_issuer_proof_verifier_support(
            &verifier_cfg
        ));
    }

    #[test]
    fn role_attestation_cache_nonroot_requires_only_root_proof_verifier_support() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: false,
            role_attestation_cache: true,
        };

        assert!(!nonroot_requires_delegated_token_issuer(&verifier_cfg));
        assert!(nonroot_requires_root_proof_verifier_support(&verifier_cfg));
        assert!(!nonroot_requires_issuer_proof_verifier_support(
            &verifier_cfg
        ));
    }

    #[test]
    fn default_nonroot_does_not_require_auth_proof_verifier_support() {
        let cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);

        assert!(!nonroot_requires_root_proof_verifier_support(&cfg));
        assert!(!nonroot_requires_issuer_proof_verifier_support(&cfg));
    }

    #[test]
    fn auth_material_nonroot_requires_the_matching_verifier_support() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: true,
            role_attestation_cache: true,
        };

        let mut issuer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        issuer_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: true,
            delegated_token_verifier: false,
            role_attestation_cache: false,
        };

        assert!(nonroot_requires_root_proof_verifier_support(&verifier_cfg));
        assert!(nonroot_requires_issuer_proof_verifier_support(
            &verifier_cfg
        ));
        assert!(nonroot_requires_root_proof_verifier_support(&issuer_cfg));
        assert!(!nonroot_requires_issuer_proof_verifier_support(&issuer_cfg));
    }

    #[cfg(not(feature = "auth-root-canister-sig-verify"))]
    #[test]
    fn delegated_token_verifier_startup_requires_canister_signature_verify_feature() {
        let _ = ConfigTestBuilder::new().install();
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: true,
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
    fn issuer_nonroot_requires_issuer_canister_signature_create() {
        let mut issuer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        issuer_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: true,
            delegated_token_verifier: false,
            role_attestation_cache: true,
        };

        assert!(nonroot_requires_delegated_token_issuer(&issuer_cfg));
    }

    #[test]
    fn runtime_auth_workflow_type_exists_for_runtime_ownership() {
        let _ = RuntimeAuthWorkflow;
    }
}
