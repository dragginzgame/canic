//! Module: ops::auth::delegated::root_proof
//!
//! Responsibility: project protected Canic context into IC Auth Root-proof verification.
//! Does not own: issuer policy, endpoint authorization, signing or proof storage.
//! Boundary: one library verifier for active-proof installation and token presentation.

#[cfg(any(feature = "auth-chain-key-ecdsa", test))]
use super::canonical::CanonicalAuthError;
use super::cert_rules::DelegatedAuthTtlLimits;
use crate::{
    cdk::types::Principal,
    dto::auth::{DelegationCert, RootProof},
    ops::auth::AuthChainKeyRootVerifierConfig,
};
use thiserror::Error;

/// Canic context and retained proof material for one Root verification.
pub(in crate::ops::auth) struct VerifyRootDelegationProofInput<'a> {
    pub cert: &'a DelegationCert,
    pub root_proof: &'a RootProof,
    pub expected_issuer: Principal,
    pub verifier: &'a AuthChainKeyRootVerifierConfig,
    pub now_ns: u64,
    pub ttl_limits: DelegatedAuthTtlLimits,
}

/// Library evidence; it neither authorizes installation nor establishes a session.
pub(in crate::ops::auth) struct VerifiedRootDelegationProof {
    pub cert_hash: [u8; 32],
    pub expires_at_ns: u64,
}

/// Canic network admission or the canonical verifier's typed refusal.
#[derive(Debug, Eq, Error, PartialEq)]
pub(in crate::ops::auth) enum RootDelegationProofError {
    #[cfg(any(feature = "auth-chain-key-ecdsa", test))]
    #[error("root key is not permitted on this network")]
    RootPolicyRejected,
    #[cfg(any(feature = "auth-chain-key-ecdsa", test))]
    #[error(transparent)]
    Verification(#[from] ic_auth::token::TokenVerificationError),
    #[cfg(any(feature = "auth-chain-key-ecdsa", test))]
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
    #[cfg(not(any(feature = "auth-chain-key-ecdsa", test)))]
    #[error("chain-key verification support is unavailable")]
    VerificationUnavailable,
}

#[cfg(any(feature = "auth-chain-key-ecdsa", test))]
pub(in crate::ops::auth) fn project_root_policy(
    policy: &crate::dto::auth::RootKeyPolicyV1,
) -> ic_auth::token::RootKeyPolicy {
    ic_auth::token::RootKeyPolicy {
        root_canister_id: policy.root_canister_id,
        algorithm: policy.algorithm,
        key_id: policy.key_id.clone(),
        derivation_path_hash: policy.derivation_path_hash,
        public_key: policy.public_key.clone(),
        key_version: policy.key_version,
        min_accepted_key_version: policy.min_accepted_key_version,
        min_accepted_proof_epoch: policy.min_accepted_proof_epoch,
        min_accepted_registry_epoch: policy.min_accepted_registry_epoch,
        valid_from_ns: policy.valid_from_ns,
        accept_until_ns: policy.accept_until_ns,
        max_revocation_latency_ns: policy.max_revocation_latency_ns,
    }
}

#[cfg(any(feature = "auth-chain-key-ecdsa", test))]
pub(in crate::ops::auth) fn verify_root_delegation_proof(
    input: VerifyRootDelegationProofInput<'_>,
) -> Result<VerifiedRootDelegationProof, RootDelegationProofError> {
    let policy = &input.verifier.policy;
    let permitted_key = match policy.build_network {
        crate::ids::BuildNetwork::Ic => policy.key_id.name == "key_1",
        crate::ids::BuildNetwork::Local => {
            policy.key_id.name != "test_key_1" || input.verifier.allow_test_chain_key
        }
    };
    if !permitted_key {
        return Err(RootDelegationProofError::RootPolicyRejected);
    }
    let root_key = project_root_policy(policy);
    let proof = super::protocol::proof_parts(input.cert, input.root_proof)?;
    let verified = ic_auth::token::verify_delegation_proof(
        &proof,
        &ic_auth::token::DelegationProofVerificationContext {
            expected_issuer: input.expected_issuer,
            root_key: &root_key,
            now_ns: input.now_ns,
            limits: ic_auth::token::DelegationProofVerificationLimits {
                max_cert_ttl_ns: input.ttl_limits.max_cert_ttl_ns,
                max_token_ttl_ns: input.ttl_limits.max_token_ttl_ns,
                max_future_skew_ns: crate::ops::auth::AUTH_TIME_SKEW_ALLOWANCE_NS,
                max_variable_bytes: 64 * 1024,
                max_witness_steps: 64,
            },
        },
    )?;
    Ok(VerifiedRootDelegationProof {
        cert_hash: verified.certificate_hash(),
        expires_at_ns: verified.expires_at_ns(),
    })
}

#[cfg(not(any(feature = "auth-chain-key-ecdsa", test)))]
pub(in crate::ops::auth) const fn verify_root_delegation_proof(
    input: VerifyRootDelegationProofInput<'_>,
) -> Result<VerifiedRootDelegationProof, RootDelegationProofError> {
    let _ = (
        input.cert,
        input.root_proof,
        input.expected_issuer,
        input.verifier,
        input.now_ns,
        input.ttl_limits,
    );
    Err(RootDelegationProofError::VerificationUnavailable)
}

#[cfg(any(feature = "auth-chain-key-ecdsa", test))]
pub(in crate::ops::auth) fn map_verification_error(
    cause: ic_auth::token::TokenVerificationError,
) -> crate::InternalError {
    use ic_auth::token::TokenVerificationError;
    match cause {
        TokenVerificationError::Expired {
            target: "root_key_policy",
        }
        | TokenVerificationError::StaleAuthority { .. }
        | TokenVerificationError::RootKeyPolicyBindingMismatch { .. }
        | TokenVerificationError::BindingMismatch {
            field: "root_public_key" | "key_version",
        } => crate::InternalError::auth_material_stale(),
        TokenVerificationError::RootSignatureBindingMismatch { .. } => {
            crate::InternalError::invalid_input()
        }
        TokenVerificationError::Expired { .. } => crate::InternalError::auth_proof_expired(),
        TokenVerificationError::NotYetValid { .. } => crate::InternalError::auth_proof_pending(),
        _ => crate::InternalError::invalid_input(),
    }
}
