//! Module: ops::auth::token::verification
//!
//! Responsibility: execute embedded-proof delegated-token verification.
//! Does not own: endpoint authorization, verifier configuration storage, or token preparation.
//! Boundary: current runtime verification context, proof callbacks, and typed metrics.

#[cfg(test)]
use crate::ops::auth::delegated::root_proof::{
    RootDelegationProofError, VerifyRootDelegationProofInput, verify_root_delegation_proof,
};
#[cfg(test)]
use crate::{
    dto::auth::{DelegationCert, RootProof},
    ops::auth::delegated::cert_rules::DelegatedAuthTtlLimits,
};

#[cfg(any(test, feature = "auth-delegated-token-verify"))]
use super::verifier_config::validate_build_network_root_key_pair;
use super::*;

#[cfg_attr(
    not(any(test, feature = "auth-delegated-token-verify")),
    expect(
        dead_code,
        reason = "protected bindings are consumed by the enabled verifier"
    )
)]
pub(super) struct DelegatedTokenLocalContext {
    local_fleet: FleetKey,
    local_role: CanisterRole,
}

pub(super) fn delegated_tokens_config_for_verification()
-> Result<DelegatedTokenConfig, InternalError> {
    let cfg = match ConfigOps::delegated_tokens_config() {
        Ok(cfg) => cfg,
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(err);
        }
    };
    if !cfg.enabled {
        DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::Disabled);
        return Err(AuthValidationError::DelegatedTokenAuthDisabled.into());
    }
    Ok(cfg)
}

pub(super) fn require_current_canister_delegated_token_verifier() -> Result<(), InternalError> {
    let canister_cfg = match ConfigOps::current_canister() {
        Ok(canister_cfg) => canister_cfg,
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(err);
        }
    };

    if canister_cfg.auth.delegated_token_verifier {
        return Ok(());
    }

    DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
    Err(AuthValidationError::Auth(
        "delegated token verifier disabled for this canister; enable auth.delegated_token_verifier on its Component Spec or child-role table in canic.toml"
            .to_string(),
    )
    .into())
}

pub(super) fn auth_proof_verifier_config_for_verification(
    cfg: &DelegatedTokenConfig,
) -> Result<AuthProofVerifierConfig, InternalError> {
    match AuthOps::auth_proof_verifier_config_from(cfg) {
        Ok(verifier_cfg) => Ok(verifier_cfg),
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::RootKey);
            Err(err)
        }
    }
}

pub(super) fn delegated_token_local_context() -> Result<DelegatedTokenLocalContext, InternalError> {
    let local_role = match EnvOps::canister_role() {
        Ok(local_role) => local_role,
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(err);
        }
    };
    let local_fleet = match FleetActivationOps::fleet_binding() {
        Ok(binding) => binding.fleet,
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(InternalError::from(StorageOpsError::from(err)));
        }
    };

    Ok(DelegatedTokenLocalContext {
        local_fleet,
        local_role,
    })
}

#[cfg(any(feature = "auth-delegated-token-verify", test))]
pub(super) fn verify_with_embedded_proofs<'a>(
    input: &'a VerifyDelegatedTokenRuntimeInput<'a>,
    ctx: &'a DelegatedTokenLocalContext,
    verifier_cfg: &'a AuthProofVerifierConfig,
) -> Result<VerifiedApplicationAuthority, InternalError> {
    validate_build_network_root_key_pair(
        verifier_cfg.build_network,
        &verifier_cfg.ic_root_public_key_raw,
    )?;
    let authority = ResolvedTokenAuthority::new(ctx, verifier_cfg)?;
    let token = protocol::token(input.token).map_err(|_| InternalError::invalid_input())?;
    let context = authority.context(
        input.caller,
        input.required_scopes,
        input.now_ns,
        input.max_cert_ttl_ns,
        input.max_token_ttl_ns,
    );
    let verified = ic_auth::token::verify_token(&token, &context).map_err(|err| {
        DelegatedAuthMetrics::record_verify_failed(shared_verification_reason(&err));
        map_shared_verification_error(err)
    })?;
    Ok(VerifiedApplicationAuthority::new(
        verified.claims().issuer_pid,
    ))
}

#[cfg(not(any(feature = "auth-delegated-token-verify", test)))]
pub(super) fn verify_with_embedded_proofs(
    _input: &VerifyDelegatedTokenRuntimeInput<'_>,
    _ctx: &DelegatedTokenLocalContext,
    _verifier_cfg: &AuthProofVerifierConfig,
) -> Result<VerifiedApplicationAuthority, InternalError> {
    Err(crate::ops::auth::AuthSignatureError::ProofUnavailable.into())
}

/// Host-resolved immutable authority; no field originates in a submitted token.
#[cfg(any(feature = "auth-delegated-token-verify", test))]
pub struct ResolvedTokenAuthority {
    pub role: ic_auth_protocol_types::AuthRole,
    pub audience: ic_auth_protocol_types::AudienceId,
    pub allowed_scopes: Vec<String>,
    pub root_key: ic_auth::token::RootKeyPolicy,
    pub ic_root_public_key_raw: Vec<u8>,
}

#[cfg(any(feature = "auth-delegated-token-verify", test))]
impl ResolvedTokenAuthority {
    fn new(
        ctx: &DelegatedTokenLocalContext,
        verifier: &AuthProofVerifierConfig,
    ) -> Result<Self, InternalError> {
        Self::from_protected(
            ctx,
            verifier,
            ConfigOps::current_canister()?.auth.allowed_token_scopes,
        )
    }

    fn from_protected(
        ctx: &DelegatedTokenLocalContext,
        verifier: &AuthProofVerifierConfig,
        allowed_scopes: Vec<String>,
    ) -> Result<Self, InternalError> {
        let policy = verifier
            .chain_key_root
            .as_ref()
            .ok_or_else(InternalError::auth_material_stale)?;
        let permitted = match policy.policy.build_network {
            BuildNetwork::Ic => policy.policy.key_id.name == "key_1",
            BuildNetwork::Local => {
                policy.policy.key_id.name != "test_key_1" || policy.allow_test_chain_key
            }
        };
        if !permitted {
            return Err(InternalError::auth_material_stale());
        }
        let root_key = crate::ops::auth::delegated::root_proof::project_root_policy(&policy.policy);
        Ok(Self {
            role: protocol::role(&ctx.local_role).map_err(|_| InternalError::invalid_input())?,
            audience: ic_auth_protocol_types::AudienceId {
                canonical_network_id: ic_auth_protocol_types::CanonicalId::from_bytes(
                    *ctx.local_fleet.canonical_network_id.as_bytes(),
                ),
                fleet_id: ic_auth_protocol_types::CanonicalId::from_bytes(
                    *ctx.local_fleet.fleet_id.as_bytes(),
                ),
            },
            allowed_scopes,
            root_key,
            ic_root_public_key_raw: verifier.ic_root_public_key_raw.clone(),
        })
    }

    pub(crate) fn context<'a>(
        &'a self,
        caller: Principal,
        required_scopes: &'a [String],
        now_ns: u64,
        max_cert_ttl_ns: u64,
        max_token_ttl_ns: u64,
    ) -> ic_auth::token::TokenVerificationContext<'a> {
        ic_auth::token::TokenVerificationContext {
            caller,
            audience: self.audience,
            role: &self.role,
            allowed_scopes: &self.allowed_scopes,
            required_scopes,
            root_key: &self.root_key,
            ic_root_public_key_raw: &self.ic_root_public_key_raw,
            now_ns,
            limits: ic_auth::token::TokenVerificationLimits {
                max_cert_ttl_ns,
                max_token_ttl_ns,
                max_future_skew_ns: crate::ops::auth::AUTH_TIME_SKEW_ALLOWANCE_NS,
                max_certificate_age_ns: 60_000_000_000,
                max_variable_bytes: 64 * 1024,
                max_issuer_signature_bytes: 128 * 1024,
                max_witness_steps: 64,
            },
        }
    }
}

#[cfg(any(feature = "auth-delegated-token-verify", test))]
pub(in crate::ops::auth) fn map_shared_verification_error(
    error: ic_auth::token::TokenVerificationError,
) -> InternalError {
    use ic_auth::token::TokenVerificationError as E;
    match error {
        E::Expired { target: "token" } => InternalError::auth_token_expired(),
        E::Expired {
            target: "root_key_policy",
        } => InternalError::auth_material_stale(),
        E::Expired { .. } => InternalError::auth_proof_expired(),
        E::NotYetValid { .. } => InternalError::auth_proof_pending(),
        E::PresenterMismatch | E::SubjectMismatch => {
            InternalError::public(crate::diagnostics::codes::AUTHORITY_CONFLICT)
        }
        E::ScopeRejected { .. } | E::RoleRejected => {
            InternalError::public(crate::diagnostics::codes::CONFIGURATION_INACTIVE)
        }
        E::AudienceRejected => InternalError::public(crate::diagnostics::codes::AUTHORITY_INACTIVE),
        E::TtlExceeded { .. } => InternalError::public(crate::diagnostics::codes::TIME_CAPACITY),
        cause => crate::ops::auth::delegated::root_proof::map_verification_error(cause),
    }
}

#[cfg(any(feature = "auth-delegated-token-verify", test))]
fn shared_verification_reason(
    error: &ic_auth::token::TokenVerificationError,
) -> DelegatedAuthMetricReason {
    use ic_auth::token::TokenVerificationError as E;
    match error {
        E::ScopeRejected { .. } => DelegatedAuthMetricReason::ScopeRejected,
        E::RoleRejected => DelegatedAuthMetricReason::TokenGrantRejected,
        E::AudienceRejected => DelegatedAuthMetricReason::Audience,
        E::Canonical(_) => DelegatedAuthMetricReason::Canonical,
        E::Expired { target: "token" } => DelegatedAuthMetricReason::TokenExpired,
        E::Expired {
            target: "certificate",
        } => DelegatedAuthMetricReason::CertExpired,
        E::Expired { .. } | E::StaleAuthority { .. } | E::RootKeyPolicyBindingMismatch { .. } => {
            DelegatedAuthMetricReason::RootKey
        }
        E::NotYetValid { target: "token" } => DelegatedAuthMetricReason::TokenNotYetValid,
        E::NotYetValid { .. } => DelegatedAuthMetricReason::CertNotYetValid,
        E::PresenterMismatch => DelegatedAuthMetricReason::PresenterCallerMismatch,
        E::SubjectMismatch => DelegatedAuthMetricReason::PresenterSubjectMismatch,
        E::BindingMismatch { field: "cert_hash" } => DelegatedAuthMetricReason::CertHashMismatch,
        E::GrantsNotSubset => DelegatedAuthMetricReason::GrantsNotSubset,
        E::TtlExceeded { .. } => DelegatedAuthMetricReason::TokenTtlExceeded,
        E::InvalidWindow { .. } => DelegatedAuthMetricReason::TokenInvalidWindow,
        E::IssuerSignature(_) => DelegatedAuthMetricReason::IssuerProofInvalid,
        _ => DelegatedAuthMetricReason::RootProofInvalid,
    }
}

impl AuthOps {
    /// Resolve a complete live verifier context for the shared session workflow.
    #[cfg(feature = "auth-local-application-authorization")]
    pub(crate) fn resolve_token_authority(
        authority: &crate::ops::auth::application_authorization::LocalApplicationAuthorizationAuthority,
    ) -> Result<ResolvedTokenAuthority, InternalError> {
        let cfg = delegated_tokens_config_for_verification()?;
        let ctx = DelegatedTokenLocalContext {
            local_fleet: authority.snapshot.fleet(),
            local_role: authority.snapshot.role().clone(),
        };
        let verifier = auth_proof_verifier_config_for_verification(&cfg)?;
        validate_build_network_root_key_pair(
            verifier.build_network,
            &verifier.ic_root_public_key_raw,
        )?;
        ResolvedTokenAuthority::from_protected(
            &ctx,
            &verifier,
            authority.allowed_token_scopes.clone(),
        )
    }

    #[cfg(test)]
    pub(crate) fn verify_delegation_root_proof(
        cert: &DelegationCert,
        root_proof: &RootProof,
        verifier_cfg: &AuthProofVerifierConfig,
        now_ns: u64,
        ttl_limits: DelegatedAuthTtlLimits,
    ) -> Result<(), InternalError> {
        let root_pid = cert.root_pid;
        if root_pid != verifier_cfg.root_canister_id {
            return Err(InternalError::invalid_input());
        }

        let Some(chain_key_root) = verifier_cfg.chain_key_root.as_ref() else {
            return Err(InternalError::auth_material_stale());
        };
        verify_root_delegation_proof(VerifyRootDelegationProofInput {
            cert,
            root_proof,
            expected_issuer: cert.issuer_pid,
            verifier: chain_key_root,
            now_ns,
            ttl_limits,
        })
        .map(|_evidence| ())
        .map_err(map_root_delegation_proof_error)
    }
}

#[cfg(test)]
pub(super) fn map_root_delegation_proof_error(cause: RootDelegationProofError) -> InternalError {
    match cause {
        RootDelegationProofError::Verification(cause) => {
            crate::ops::auth::delegated::root_proof::map_verification_error(cause)
        }
        _ => InternalError::invalid_input(),
    }
}
