//! Module: workflow::runtime::auth
//!
//! Responsibility: orchestrate runtime auth startup checks and local verification.
//! Does not own: endpoint authorization, auth storage records, or crypto primitives.
//! Boundary: lifecycle and API layers call this after config/runtime context is available.

mod prepare;
mod provisioning;
mod renewal;
mod root_delegation_batch;
mod root_issuer;

use crate::{
    InternalError,
    cdk::types::Principal,
    config::ConfigModel,
    domain::policy::pure::auth::application_authorization::{
        ApplicationAuthorityBindingTransition, decide_application_authority_binding_transition,
    },
    dto::auth::SignedRoleAttestation,
    format::display_optional,
    ids::{CanisterRole, ManagedCanisterBinding},
    log,
    log::Topic,
    model::auth::application_authorization::LocalApplicationAuthorityBinding,
    ops::{
        auth::{AuthExpiryError, AuthOps, AuthOpsError},
        config::{ConfigOps, RootConfigOps},
        ic::IcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            record_attestation_epoch_rejected, record_attestation_verify_failed,
        },
        storage::auth::AuthStateOps,
    },
    workflow::runtime::fleet_activation::FleetActivationWorkflow,
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
    /// Reconcile the one locally activated application authority binding.
    pub fn reconcile_local_application_authority()
    -> Result<ApplicationAuthorityBindingTransition, InternalError> {
        let binding = AuthOps::local_application_authority_binding()?;
        Self::reconcile_application_authority_binding(binding)
    }

    fn reconcile_application_authority_binding(
        binding: LocalApplicationAuthorityBinding,
    ) -> Result<ApplicationAuthorityBindingTransition, InternalError> {
        let previous = AuthStateOps::application_authority_binding()
            .map_err(|_| InternalError::invariant())?;
        let transition =
            decide_application_authority_binding_transition(previous.as_ref(), &binding);
        match transition {
            ApplicationAuthorityBindingTransition::AdvanceGeneration => {
                AuthStateOps::advance_application_authority_binding_generation(binding)
                    .map_err(|_| InternalError::invariant())?;
            }
            ApplicationAuthorityBindingTransition::Initialize
            | ApplicationAuthorityBindingTransition::UpdateWithoutGeneration => {
                AuthStateOps::set_application_authority_binding(binding)
                    .map_err(|_| InternalError::invariant())?;
            }
            ApplicationAuthorityBindingTransition::Unchanged => {}
        }
        Ok(transition)
    }

    /// Return the exact root issuer-renewal native identity.
    pub(crate) fn root_issuer_renewal_timer_identity()
    -> Result<ic_timers::TimerIdentity, crate::workflow::runtime::timer::TimerError> {
        renewal::RootIssuerRenewalWorkflow::timer_identity()
    }

    /// Return the claimed root issuer-renewal identity, when declared.
    pub(crate) fn claimed_root_issuer_renewal_timer_identity()
    -> Result<Option<ic_timers::TimerIdentity>, crate::workflow::runtime::timer::TimerError> {
        renewal::RootIssuerRenewalWorkflow::claimed_timer_identity()
    }

    /// Cancel the retained root issuer-renewal registration for snapshot suspension.
    pub(crate) fn cancel_root_issuer_renewal_timer()
    -> Result<(), crate::workflow::runtime::timer::TimerError> {
        renewal::RootIssuerRenewalWorkflow::cancel_timer()
    }

    /// Recover one expired root issuer-renewal attempt from authoritative auth demand.
    pub(crate) fn recover_expired_root_issuer_renewal(now_ns: u64) -> bool {
        renewal::RootIssuerRenewalWorkflow::recover_expired(now_ns)
    }

    /// Reconstruct or update the root issuer renewal deadline from durable authority.
    pub fn reconcile_root_issuer_renewal() -> Result<(), InternalError> {
        renewal::RootIssuerRenewalWorkflow::reconcile()
    }

    /// Fail fast when root delegated-auth config requires missing crypto support.
    pub fn ensure_root_crypto_contract() -> Result<(), InternalError> {
        let cfg = RootConfigOps::get()?;
        if root_requires_role_attestation_proofs(&cfg)
            && !AuthOps::root_canister_sig_create_enabled()
        {
            return Err(InternalError::invariant());
        }

        if AuthOps::has_enabled_root_issuer_renewal_templates()
            && !AuthOps::chain_key_root_sign_enabled()
        {
            return Err(InternalError::invariant());
        }

        Ok(())
    }

    /// Fail fast when one delegated-token issuer lacks canister-signature support.
    pub fn ensure_nonroot_crypto_contract(
        canister_role: &CanisterRole,
        canister_cfg: &crate::config::RuntimeCanisterConfig,
    ) -> Result<(), InternalError> {
        if nonroot_requires_delegated_token_issuer(&canister_cfg.auth)
            && !AuthOps::issuer_canister_sig_create_enabled()
        {
            return Err(InternalError::invariant());
        }

        Self::ensure_auth_proof_verifier_support_contract(canister_role, &canister_cfg.auth)?;

        Ok(())
    }

    /// Fail fast when a non-root auth verifier lacks hard-cut trust anchors.
    fn ensure_auth_proof_verifier_support_contract(
        _canister_role: &CanisterRole,
        canister_auth: &crate::config::schema::CanisterAuthConfig,
    ) -> Result<(), InternalError> {
        let delegated_tokens_cfg = ConfigOps::delegated_tokens_config()?;
        if !nonroot_requires_root_proof_verifier_support(canister_auth) {
            return Ok(());
        }

        if canister_auth.role_attestation_cache && !AuthOps::root_canister_sig_verify_enabled() {
            return Err(InternalError::invariant());
        }

        if nonroot_requires_issuer_proof_verifier_support(canister_auth)
            && !AuthOps::issuer_canister_sig_verify_enabled()
        {
            return Err(InternalError::invariant());
        }

        if nonroot_requires_chain_key_root_proof_support(canister_auth)
            && !AuthOps::chain_key_ecdsa_enabled()
        {
            return Err(InternalError::invariant());
        }

        if delegated_tokens_cfg.enabled || canister_auth.role_attestation_cache {
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
        let context = role_attestation_verification_context(attestation, min_accepted_epoch)?;
        let verifier_subnet = Some(role_attestation_verifier_subnet()?);
        let result = AuthOps::verify_role_attestation_cached(
            attestation,
            context.caller,
            context.self_pid,
            verifier_subnet,
            context.now_ns,
            context.min_accepted_epoch,
        )
        .map(|_| ());
        finish_role_attestation_verification(result, attestation, context)
    }

    /// Require a root-signed role attestation bound to the receiver's live Subnet.
    pub async fn verify_local_subnet_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), InternalError> {
        // Proof verification and Subnet comparison are local runtime work.
        std::future::ready(()).await;
        let context = role_attestation_verification_context(attestation, min_accepted_epoch)?;
        let result = AuthOps::verify_local_subnet_role_attestation_cached(
            attestation,
            context.caller,
            context.self_pid,
            IcOps::subnet_self(),
            context.now_ns,
            context.min_accepted_epoch,
        )
        .map(|_| ());
        finish_role_attestation_verification(result, attestation, context)
    }
}

#[derive(Clone, Copy)]
struct RoleAttestationVerificationContext {
    caller: Principal,
    self_pid: Principal,
    now_ns: u64,
    min_accepted_epoch: u64,
}

fn role_attestation_verification_context(
    attestation: &SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<RoleAttestationVerificationContext, InternalError> {
    let configured_min_accepted_epoch = ConfigOps::role_attestation_config()?
        .min_accepted_epoch_by_role
        .get(attestation.payload.role.as_str())
        .copied();

    Ok(RoleAttestationVerificationContext {
        caller: IcOps::msg_caller(),
        self_pid: IcOps::canister_self(),
        now_ns: IcOps::now_nanos(),
        min_accepted_epoch: resolve_min_accepted_epoch(
            min_accepted_epoch,
            configured_min_accepted_epoch,
        ),
    })
}

fn finish_role_attestation_verification(
    result: Result<(), AuthOpsError>,
    attestation: &SignedRoleAttestation,
    context: RoleAttestationVerificationContext,
) -> Result<(), InternalError> {
    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            record_attestation_verifier_rejection(&err);
            log_attestation_verifier_rejection(&err, attestation, context.caller, context.self_pid);
            Err(err.into())
        }
    }
}

fn role_attestation_verifier_subnet() -> Result<Principal, InternalError> {
    if EnvOps::is_root() {
        return Ok(FleetActivationWorkflow::root_authority()?
            .binding
            .placement_subnet
            .into_principal());
    }

    let binding = EnvOps::managed_binding()?;
    let component = match &binding {
        ManagedCanisterBinding::Component(component) => component,
        ManagedCanisterBinding::ComponentChild(child) => &child.component,
    };
    Ok(component.placement_subnet.into_principal())
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

fn root_requires_role_attestation_proofs(cfg: &ConfigModel) -> bool {
    cfg.component_specs.values().any(|component_spec| {
        component_spec.auth.role_attestation_cache
            || component_spec
                .children
                .values()
                .any(|child| child.auth.role_attestation_cache)
    })
}

const fn nonroot_requires_delegated_token_issuer(
    auth: &crate::config::schema::CanisterAuthConfig,
) -> bool {
    auth.delegated_token_issuer
}

const fn nonroot_requires_root_proof_verifier_support(
    auth: &crate::config::schema::CanisterAuthConfig,
) -> bool {
    auth.delegated_token_issuer || auth.delegated_token_verifier || auth.role_attestation_cache
}

const fn nonroot_requires_issuer_proof_verifier_support(
    auth: &crate::config::schema::CanisterAuthConfig,
) -> bool {
    auth.delegated_token_verifier
}

const fn nonroot_requires_chain_key_root_proof_support(
    auth: &crate::config::schema::CanisterAuthConfig,
) -> bool {
    auth.delegated_token_issuer || auth.delegated_token_verifier
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeAuthWorkflow, nonroot_requires_delegated_token_issuer,
        nonroot_requires_issuer_proof_verifier_support,
        nonroot_requires_root_proof_verifier_support, root_requires_role_attestation_proofs,
    };
    use crate::{
        cdk::types::Principal,
        config::schema::{CanisterAuthConfig, CanisterKind},
        domain::policy::pure::auth::application_authorization::ApplicationAuthorityBindingTransition,
        ids::CanisterRole,
        model::auth::application_authorization::{
            ApplicationScope, CanonicalApplicationScopes, LocalApplicationAuthorityBinding,
        },
        ops::storage::auth::{
            AuthStateOps, application_sessions::ApplicationSessionTestStateGuard,
        },
        test::{config::ConfigTestBuilder, seams, support::fleet_key},
    };

    fn application_authority_binding(
        scopes: &[&str],
        maximum_session_ttl_secs: u64,
    ) -> LocalApplicationAuthorityBinding {
        let scopes = CanonicalApplicationScopes::for_verified_grant(
            scopes
                .iter()
                .map(|scope| ApplicationScope::parse(*scope).unwrap())
                .collect(),
        )
        .unwrap();
        LocalApplicationAuthorityBinding::enabled(
            fleet_key(1),
            CanisterRole::new("component"),
            Principal::from_slice(&[9; 29]),
            Some(4),
            scopes,
            maximum_session_ttl_secs,
        )
    }

    #[test]
    fn application_authority_reconciliation_composes_policy_and_storage_mutation() {
        let _lock = seams::lock();
        let _state = ApplicationSessionTestStateGuard::empty();
        let original = application_authority_binding(&["app:read"], 900);
        assert_eq!(
            RuntimeAuthWorkflow::reconcile_application_authority_binding(original).unwrap(),
            ApplicationAuthorityBindingTransition::Initialize
        );
        assert_eq!(AuthStateOps::application_authority_generation(), 0);

        let expanded = application_authority_binding(&["app:read", "app:write"], 1_000);
        assert_eq!(
            RuntimeAuthWorkflow::reconcile_application_authority_binding(expanded).unwrap(),
            ApplicationAuthorityBindingTransition::UpdateWithoutGeneration
        );
        assert_eq!(AuthStateOps::application_authority_generation(), 0);

        let narrowed = application_authority_binding(&["app:read"], 900);
        assert_eq!(
            RuntimeAuthWorkflow::reconcile_application_authority_binding(narrowed.clone()).unwrap(),
            ApplicationAuthorityBindingTransition::AdvanceGeneration
        );
        assert_eq!(AuthStateOps::application_authority_generation(), 1);
        assert_eq!(
            RuntimeAuthWorkflow::reconcile_application_authority_binding(narrowed).unwrap(),
            ApplicationAuthorityBindingTransition::Unchanged
        );
        assert_eq!(AuthStateOps::application_authority_generation(), 1);
    }

    #[test]
    fn root_does_not_require_canister_signature_proofs_for_delegated_issuer_when_enabled() {
        let mut issuer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        issuer_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: true,
            delegated_token_verifier: false,
            local_application_authorization: None,
            role_attestation_cache: false,
        };

        let cfg = ConfigTestBuilder::new()
            .with_default_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_default_canister("user_shard", issuer_cfg)
            .build();

        assert!(!root_requires_role_attestation_proofs(&cfg));
    }

    #[test]
    fn root_requires_canister_signature_proofs_for_role_attestation_cache_when_delegated_tokens_disabled()
     {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: false,
            local_application_authorization: None,
            role_attestation_cache: true,
        };

        let mut cfg = ConfigTestBuilder::new()
            .with_default_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_default_canister("project_hub", verifier_cfg)
            .build();
        cfg.auth.delegated_tokens.enabled = false;

        assert!(root_requires_role_attestation_proofs(&cfg));
    }

    #[test]
    fn root_ignores_delegated_issuer_when_delegated_tokens_disabled() {
        let mut issuer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        issuer_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: true,
            delegated_token_verifier: false,
            local_application_authorization: None,
            role_attestation_cache: false,
        };

        let mut cfg = ConfigTestBuilder::new()
            .with_default_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_default_canister("user_shard", issuer_cfg)
            .build();
        cfg.auth.delegated_tokens.enabled = false;

        assert!(!root_requires_role_attestation_proofs(&cfg));
    }

    #[test]
    fn root_does_not_require_auth_crypto_without_auth_roles() {
        let cfg = ConfigTestBuilder::new().build();

        assert!(!root_requires_role_attestation_proofs(&cfg));
    }

    #[test]
    fn verifier_only_nonroot_requires_chain_key_root_and_issuer_proof_verifier_support() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: true,
            local_application_authorization: None,
            role_attestation_cache: false,
        };

        assert!(!nonroot_requires_delegated_token_issuer(&verifier_cfg.auth));
        assert!(nonroot_requires_root_proof_verifier_support(
            &verifier_cfg.auth
        ));
        assert!(nonroot_requires_issuer_proof_verifier_support(
            &verifier_cfg.auth
        ));
    }

    #[test]
    fn role_attestation_cache_nonroot_requires_only_root_proof_verifier_support() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: false,
            local_application_authorization: None,
            role_attestation_cache: true,
        };

        assert!(!nonroot_requires_delegated_token_issuer(&verifier_cfg.auth));
        assert!(nonroot_requires_root_proof_verifier_support(
            &verifier_cfg.auth
        ));
        assert!(!nonroot_requires_issuer_proof_verifier_support(
            &verifier_cfg.auth
        ));
    }

    #[test]
    fn default_nonroot_does_not_require_auth_proof_verifier_support() {
        let cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);

        assert!(!nonroot_requires_root_proof_verifier_support(&cfg.auth));
        assert!(!nonroot_requires_issuer_proof_verifier_support(&cfg.auth));
    }

    #[test]
    fn auth_material_nonroot_requires_the_matching_verifier_support() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: true,
            local_application_authorization: None,
            role_attestation_cache: true,
        };

        let mut issuer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        issuer_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: true,
            delegated_token_verifier: false,
            local_application_authorization: None,
            role_attestation_cache: false,
        };

        assert!(nonroot_requires_root_proof_verifier_support(
            &verifier_cfg.auth
        ));
        assert!(nonroot_requires_issuer_proof_verifier_support(
            &verifier_cfg.auth
        ));
        assert!(nonroot_requires_root_proof_verifier_support(
            &issuer_cfg.auth
        ));
        assert!(!nonroot_requires_issuer_proof_verifier_support(
            &issuer_cfg.auth
        ));
    }

    #[cfg(not(feature = "auth-root-canister-sig-verify"))]
    #[test]
    fn role_attestation_cache_startup_requires_root_canister_signature_verify_feature() {
        let _ = ConfigTestBuilder::new().install();
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: true,
            local_application_authorization: None,
            role_attestation_cache: true,
        };
        let role = CanisterRole::new("app");

        RuntimeAuthWorkflow::ensure_nonroot_crypto_contract(&role, &verifier_cfg.into())
            .expect_err("expected verifier feature error");
    }

    #[test]
    fn issuer_nonroot_requires_issuer_canister_signature_create() {
        let mut issuer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        issuer_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer: true,
            delegated_token_verifier: false,
            local_application_authorization: None,
            role_attestation_cache: true,
        };

        assert!(nonroot_requires_delegated_token_issuer(&issuer_cfg.auth));
    }

    #[test]
    fn runtime_auth_workflow_type_exists_for_runtime_ownership() {
        let _ = RuntimeAuthWorkflow;
    }
}
