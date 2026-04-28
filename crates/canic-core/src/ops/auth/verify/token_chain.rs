use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegationCert, DelegationProof},
    ops::{
        auth::{
            DelegationExpiryError, DelegationSignatureError, TokenAudience, TokenGrant,
            TokenLifetime, VerifiedTokenClaims,
        },
        config::ConfigOps,
        ic::{IcOps, ecdsa::EcdsaOps},
        runtime::env::EnvOps,
        runtime::metrics::auth::record_verifier_cert_expired,
        storage::auth::DelegationStateOps,
    },
};

use super::proof_state::verify_current_proof;
use crate::ops::auth::audience;
use crate::ops::auth::crypto;

enum SignatureKind {
    Delegation,
    Token,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TokenTrustChainStage {
    Structure,
    CurrentProof,
    DelegationSignature,
    TokenSignature,
}

impl TokenTrustChainStage {
    #[cfg(test)]
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Structure => "structure",
            Self::CurrentProof => "current_proof",
            Self::DelegationSignature => "delegation_signature",
            Self::TokenSignature => "token_signature",
        }
    }
}

///
/// TokenTrustChainContext
///

#[derive(Clone, Copy)]
struct TokenTrustChainContext {
    authority_pid: Principal,
    now_secs: u64,
    self_audience_pid: Option<Principal>,
}

// Translate low-level ECDSA verification failures into trust-chain error variants.
fn map_signature_error(err: InternalError, kind: SignatureKind) -> InternalError {
    match kind {
        SignatureKind::Delegation => {
            DelegationSignatureError::CertSignatureInvalid(err.to_string()).into()
        }
        SignatureKind::Token => {
            DelegationSignatureError::TokenSignatureInvalid(err.to_string()).into()
        }
    }
}

// Run the canonical delegated-token trust chain in one place so stage order cannot drift.
pub(super) fn verify_token_trust_chain(
    token: &DelegatedToken,
    authority_pid: Principal,
    now_secs: u64,
    self_pid: Principal,
) -> Result<(), InternalError> {
    verify_token_trust_chain_with_probe_and_steps(
        token,
        TokenTrustChainContext {
            authority_pid,
            now_secs,
            self_audience_pid: Some(self_pid),
        },
        |_| {},
        verify_current_proof,
        verify_delegation_signature,
        verify_token_sig,
    )
}

// Verify an issuer-side token trust chain without requiring the old audience
// to include the local signer.
pub(super) fn verify_token_trust_chain_for_reissue(
    token: &DelegatedToken,
    authority_pid: Principal,
    now_secs: u64,
) -> Result<(), InternalError> {
    verify_token_trust_chain_with_probe_and_steps(
        token,
        TokenTrustChainContext {
            authority_pid,
            now_secs,
            self_audience_pid: None,
        },
        |_| {},
        verify_current_proof,
        verify_delegation_signature,
        verify_token_sig,
    )
}

// Execute the trust-chain stages while preserving a test seam for order tracing.
fn verify_token_trust_chain_with_probe_and_steps<F, CurrentProof, DelegationSig, TokenSig>(
    token: &DelegatedToken,
    ctx: TokenTrustChainContext,
    mut on_stage: F,
    verify_current_proof_step: CurrentProof,
    verify_delegation_signature_step: DelegationSig,
    verify_token_sig_step: TokenSig,
) -> Result<(), InternalError>
where
    F: FnMut(TokenTrustChainStage),
    CurrentProof: FnOnce(&DelegationProof) -> Result<(), InternalError>,
    DelegationSig: FnOnce(&DelegationProof) -> Result<(), InternalError>,
    TokenSig: FnOnce(&DelegatedToken) -> Result<(), InternalError>,
{
    on_stage(TokenTrustChainStage::Structure);
    crate::ops::auth::DelegatedTokenOps::verify_delegation_structure(
        &token.proof,
        Some(ctx.authority_pid),
    )?;
    let claims = VerifiedTokenClaims::from_dto_ref(&token.claims);
    verify_time_bounds(claims.lifetime(), &token.proof.cert, ctx.now_secs)?;
    validate_claims_against_cert(claims.grant(), &token.proof.cert)?;
    if let Some(self_pid) = ctx.self_audience_pid {
        verify_self_audience(claims.audience(), self_pid)?;
    }

    on_stage(TokenTrustChainStage::CurrentProof);
    verify_current_proof_step(&token.proof)?;

    on_stage(TokenTrustChainStage::DelegationSignature);
    verify_delegation_signature_step(&token.proof)?;

    on_stage(TokenTrustChainStage::TokenSignature);
    verify_token_sig_step(token)?;

    Ok(())
}

// Trace the ordered trust-chain stages for a valid or invalid token path in tests.
#[cfg(test)]
pub(super) fn trace_token_trust_chain(
    token: &DelegatedToken,
    authority_pid: Principal,
    now_secs: u64,
    self_pid: Principal,
) -> (Vec<&'static str>, Result<(), InternalError>) {
    let mut stages = Vec::new();
    let result = verify_token_trust_chain_with_probe_and_steps(
        token,
        TokenTrustChainContext {
            authority_pid,
            now_secs,
            self_audience_pid: Some(self_pid),
        },
        |stage| {
            stages.push(stage.label());
        },
        verify_current_proof,
        verify_delegation_signature,
        verify_token_sig,
    );
    (stages, result)
}

// Force a current-proof failure in tests to prove signatures never run first.
#[cfg(test)]
pub(super) fn trace_token_trust_chain_with_forced_current_proof_failure(
    token: &DelegatedToken,
    authority_pid: Principal,
    now_secs: u64,
    self_pid: Principal,
    err: InternalError,
) -> (Vec<&'static str>, Result<(), InternalError>) {
    let mut stages = Vec::new();
    let result = verify_token_trust_chain_with_probe_and_steps(
        token,
        TokenTrustChainContext {
            authority_pid,
            now_secs,
            self_audience_pid: Some(self_pid),
        },
        |stage| {
            stages.push(stage.label());
        },
        move |_| Err(err),
        verify_delegation_signature,
        verify_token_sig,
    );
    (stages, result)
}

// Verify the root-signed delegation certificate against the cached root public key.
pub(super) fn verify_delegation_signature(proof: &DelegationProof) -> Result<(), InternalError> {
    if proof.cert_sig.is_empty() {
        return Err(DelegationSignatureError::CertSignatureUnavailable.into());
    }

    let root_public_key = DelegationStateOps::root_public_key()
        .ok_or(DelegationSignatureError::RootPublicKeyUnavailable)?;
    let hash = crypto::cert_hash(&proof.cert);
    EcdsaOps::verify_signature(&root_public_key, hash, &proof.cert_sig)
        .map_err(|err| map_signature_error(err, SignatureKind::Delegation))?;

    Ok(())
}

// Verify the shard-signed token signature against the cached shard public key.
pub(super) fn verify_token_sig(token: &DelegatedToken) -> Result<(), InternalError> {
    if token.token_sig.is_empty() {
        return Err(DelegationSignatureError::TokenSignatureUnavailable.into());
    }

    let shard_public_key = DelegationStateOps::shard_public_key(token.proof.cert.shard_pid).ok_or(
        DelegationSignatureError::ShardPublicKeyUnavailable {
            shard_pid: token.proof.cert.shard_pid,
        },
    )?;

    let claims = VerifiedTokenClaims::from_dto_ref(&token.claims);
    let token_hash = crypto::token_signing_hash(&claims, &token.proof.cert)?;
    EcdsaOps::verify_signature(&shard_public_key, token_hash, &token.token_sig)
        .map_err(|err| map_signature_error(err, SignatureKind::Token))?;

    Ok(())
}

// Enforce token timing bounds relative to both token claims and the delegation cert.
pub(super) fn verify_time_bounds(
    lifetime: TokenLifetime,
    cert: &DelegationCert,
    now_secs: u64,
) -> Result<(), InternalError> {
    if lifetime.exp < lifetime.iat {
        return Err(DelegationExpiryError::TokenExpiryBeforeIssued.into());
    }

    if now_secs < lifetime.iat {
        return Err(DelegationExpiryError::TokenNotYetValid { iat: lifetime.iat }.into());
    }

    if now_secs > lifetime.exp {
        return Err(DelegationExpiryError::TokenExpired { exp: lifetime.exp }.into());
    }

    if now_secs > cert.expires_at {
        record_verifier_cert_expired();
        let local = IcOps::canister_self();
        crate::log!(
            crate::log::Topic::Auth,
            Warn,
            "delegation cert expired local={} shard={} now_secs={} expires_at={}",
            local,
            cert.shard_pid,
            now_secs,
            cert.expires_at
        );
        return Err(DelegationExpiryError::CertExpired {
            expires_at: cert.expires_at,
        }
        .into());
    }

    if lifetime.iat < cert.issued_at {
        return Err(DelegationExpiryError::TokenIssuedBeforeDelegation {
            token_iat: lifetime.iat,
            cert_iat: cert.issued_at,
        }
        .into());
    }

    if lifetime.exp > cert.expires_at {
        return Err(DelegationExpiryError::TokenOutlivesDelegation {
            token_exp: lifetime.exp,
            cert_exp: cert.expires_at,
        }
        .into());
    }

    Ok(())
}

// Enforce the configured delegated-token max TTL bound before deeper verification.
pub(super) fn verify_max_ttl(
    lifetime: TokenLifetime,
    max_ttl_secs: u64,
) -> Result<(), DelegationExpiryError> {
    let ttl_secs = lifetime
        .exp
        .checked_sub(lifetime.iat)
        .ok_or(DelegationExpiryError::TokenExpiryBeforeIssued)?;
    if ttl_secs > max_ttl_secs {
        return Err(DelegationExpiryError::TokenTtlExceeded {
            ttl_secs,
            max_ttl_secs,
        });
    }

    Ok(())
}

// Require the verifier canister to be explicitly present in the token audience.
pub(super) fn verify_self_audience(
    audience_input: TokenAudience<'_>,
    self_pid: Principal,
) -> Result<(), InternalError> {
    let self_role = EnvOps::canister_role()?;
    let self_is_verifier =
        self_is_root_runtime() || ConfigOps::current_canister()?.delegated_auth.verifier;
    audience::verify_self_audience(audience_input, self_pid, &self_role, self_is_verifier)
        .map_err(InternalError::from)
}

// Return whether the executing wasm canister is the configured root.
#[cfg(target_arch = "wasm32")]
fn self_is_root_runtime() -> bool {
    EnvOps::is_root()
}

// Keep host-side unit tests away from IC-only `canister_self`.
#[cfg(not(target_arch = "wasm32"))]
const fn self_is_root_runtime() -> bool {
    false
}

// Enforce token grant bounds against the delegation certificate scope and audience.
pub(super) fn validate_claims_against_cert(
    grant: TokenGrant<'_>,
    cert: &DelegationCert,
) -> Result<(), InternalError> {
    audience::validate_claims_against_cert(grant, cert).map_err(InternalError::from)
}
