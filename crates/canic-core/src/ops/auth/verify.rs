use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{
        AttestationKey, DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof,
        RoleAttestation,
    },
    ops::{
        auth::{
            DelegatedTokenOpsError, DelegationExpiryError, DelegationScopeError,
            DelegationSignatureError, DelegationValidationError,
        },
        ic::{IcOps, ecdsa::EcdsaOps},
        runtime::metrics::auth::{
            record_verifier_cert_expired, record_verifier_proof_cache_stats,
            record_verifier_proof_mismatch, record_verifier_proof_miss,
        },
        storage::auth::DelegationStateOps,
    },
};

use super::crypto;

enum SignatureKind {
    Delegation,
    Token,
}

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

pub(super) fn verify_delegation_signature(proof: &DelegationProof) -> Result<(), InternalError> {
    if proof.cert_sig.is_empty() {
        return Err(DelegationSignatureError::CertSignatureUnavailable.into());
    }

    let root_public_key = DelegationStateOps::root_public_key()
        .ok_or(DelegationSignatureError::RootPublicKeyUnavailable)?;
    let hash = crypto::cert_hash(&proof.cert)?;
    EcdsaOps::verify_signature(&root_public_key, hash, &proof.cert_sig)
        .map_err(|err| map_signature_error(err, SignatureKind::Delegation))?;

    Ok(())
}

pub(super) fn verify_token_sig(token: &DelegatedToken) -> Result<(), InternalError> {
    if token.token_sig.is_empty() {
        return Err(DelegationSignatureError::TokenSignatureUnavailable.into());
    }

    let shard_public_key = DelegationStateOps::shard_public_key(token.proof.cert.shard_pid).ok_or(
        DelegationSignatureError::ShardPublicKeyUnavailable {
            shard_pid: token.proof.cert.shard_pid,
        },
    )?;

    let token_hash = crypto::token_signing_hash(&token.claims, &token.proof.cert)?;
    EcdsaOps::verify_signature(&shard_public_key, token_hash, &token.token_sig)
        .map_err(|err| map_signature_error(err, SignatureKind::Token))?;

    Ok(())
}

pub(super) fn verify_time_bounds(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
    now_secs: u64,
) -> Result<(), InternalError> {
    if claims.exp < claims.iat {
        return Err(DelegationExpiryError::TokenExpiryBeforeIssued.into());
    }

    if now_secs < claims.iat {
        return Err(DelegationExpiryError::TokenNotYetValid { iat: claims.iat }.into());
    }

    if now_secs > claims.exp {
        return Err(DelegationExpiryError::TokenExpired { exp: claims.exp }.into());
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

    if claims.iat < cert.issued_at {
        return Err(DelegationExpiryError::TokenIssuedBeforeDelegation {
            token_iat: claims.iat,
            cert_iat: cert.issued_at,
        }
        .into());
    }

    if claims.exp > cert.expires_at {
        return Err(DelegationExpiryError::TokenOutlivesDelegation {
            token_exp: claims.exp,
            cert_exp: cert.expires_at,
        }
        .into());
    }

    Ok(())
}

pub(super) fn verify_current_proof(proof: &DelegationProof) -> Result<(), InternalError> {
    let now_secs = IcOps::now_secs();
    let Some(stored) = DelegationStateOps::matching_proof_dto(proof)? else {
        let stats = DelegationStateOps::proof_cache_stats(now_secs)?;
        record_verifier_proof_cache_stats(
            stats.size,
            stats.active_count,
            stats.capacity,
            stats.profile,
            stats.active_window_secs,
        );
        record_verifier_proof_miss();
        let local = IcOps::canister_self();
        crate::log!(
            crate::log::Topic::Auth,
            Warn,
            "delegation proof miss local={} shard={}",
            local,
            proof.cert.shard_pid
        );
        return Err(DelegationValidationError::ProofMiss.into());
    };

    if proofs_equal(proof, &stored) {
        let _ = DelegationStateOps::mark_matching_proof_verified(proof, now_secs)?;
        let stats = DelegationStateOps::proof_cache_stats(now_secs)?;
        record_verifier_proof_cache_stats(
            stats.size,
            stats.active_count,
            stats.capacity,
            stats.profile,
            stats.active_window_secs,
        );
        Ok(())
    } else {
        let stats = DelegationStateOps::proof_cache_stats(now_secs)?;
        record_verifier_proof_cache_stats(
            stats.size,
            stats.active_count,
            stats.capacity,
            stats.profile,
            stats.active_window_secs,
        );
        record_verifier_proof_mismatch();
        let local = IcOps::canister_self();
        crate::log!(
            crate::log::Topic::Auth,
            Warn,
            "delegation proof mismatch local={} shard={} stored_shard={}",
            local,
            proof.cert.shard_pid,
            stored.cert.shard_pid
        );
        Err(DelegationValidationError::ProofMismatch.into())
    }
}

pub(super) fn proofs_equal(a: &DelegationProof, b: &DelegationProof) -> bool {
    let a_cert = &a.cert;
    let b_cert = &b.cert;

    a_cert.root_pid == b_cert.root_pid
        && a_cert.shard_pid == b_cert.shard_pid
        && a_cert.issued_at == b_cert.issued_at
        && a_cert.expires_at == b_cert.expires_at
        && a_cert.scopes == b_cert.scopes
        && a_cert.aud == b_cert.aud
        && a.cert_sig == b.cert_sig
}

pub(super) fn verify_max_ttl(
    token: &DelegatedToken,
    max_ttl_secs: u64,
) -> Result<(), InternalError> {
    let ttl_secs = token.claims.exp - token.claims.iat;
    if ttl_secs > max_ttl_secs {
        return Err(DelegationExpiryError::TokenTtlExceeded {
            ttl_secs,
            max_ttl_secs,
        }
        .into());
    }

    Ok(())
}

pub(super) fn verify_self_audience(
    claims: &DelegatedTokenClaims,
    self_pid: Principal,
) -> Result<(), InternalError> {
    if claims.aud.contains(&self_pid) {
        Ok(())
    } else {
        Err(DelegationScopeError::SelfAudienceMissing { self_pid }.into())
    }
}

pub(super) fn validate_claims_against_cert(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
) -> Result<(), InternalError> {
    if claims.shard_pid != cert.shard_pid {
        return Err(DelegationScopeError::ShardPidMismatch {
            expected: cert.shard_pid,
            found: claims.shard_pid,
        }
        .into());
    }

    for aud in &claims.aud {
        if !cert.aud.iter().any(|allowed| allowed == aud) {
            return Err(DelegationScopeError::AudienceNotAllowed { aud: *aud }.into());
        }
    }

    for scope in &claims.scopes {
        if !cert.scopes.iter().any(|allowed| allowed == scope) {
            return Err(DelegationScopeError::ScopeNotAllowed {
                scope: scope.clone(),
            }
            .into());
        }
    }

    Ok(())
}

pub(super) fn verify_role_attestation_claims(
    payload: &RoleAttestation,
    caller: Principal,
    self_pid: Principal,
    verifier_subnet: Option<Principal>,
    now_secs: u64,
    min_accepted_epoch: u64,
) -> Result<(), DelegatedTokenOpsError> {
    if payload.subject != caller {
        return Err(DelegationScopeError::AttestationSubjectMismatch {
            expected: caller,
            found: payload.subject,
        }
        .into());
    }

    if now_secs > payload.expires_at {
        return Err(DelegationExpiryError::AttestationExpired {
            expires_at: payload.expires_at,
            now_secs,
        }
        .into());
    }

    if let Some(audience) = payload.audience
        && audience != self_pid
    {
        return Err(DelegationScopeError::AttestationAudienceMismatch {
            expected: self_pid,
            found: audience,
        }
        .into());
    }

    if let Some(attestation_subnet) = payload.subnet_id {
        let verifier_subnet =
            verifier_subnet.ok_or(DelegationValidationError::AttestationSubnetUnavailable)?;
        if attestation_subnet != verifier_subnet {
            return Err(DelegationScopeError::AttestationSubnetMismatch {
                expected: verifier_subnet,
                found: attestation_subnet,
            }
            .into());
        }
    }

    if payload.epoch < min_accepted_epoch {
        return Err(DelegationExpiryError::AttestationEpochRejected {
            epoch: payload.epoch,
            min_accepted_epoch,
        }
        .into());
    }

    Ok(())
}

pub(super) fn verify_attestation_key_validity(
    key: &AttestationKey,
    now_secs: u64,
) -> Result<(), DelegatedTokenOpsError> {
    if let Some(valid_from) = key.valid_from
        && now_secs < valid_from
    {
        return Err(DelegationExpiryError::AttestationKeyNotYetValid {
            key_id: key.key_id,
            valid_from,
            now_secs,
        }
        .into());
    }

    if let Some(valid_until) = key.valid_until
        && now_secs > valid_until
    {
        return Err(DelegationExpiryError::AttestationKeyExpired {
            key_id: key.key_id,
            valid_until,
            now_secs,
        }
        .into());
    }

    Ok(())
}
