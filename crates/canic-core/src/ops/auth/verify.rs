use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{
        AttestationKey, DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof,
        RoleAttestation,
    },
    ops::{
        auth::DelegatedTokenOpsError,
        ic::{IcOps, ecdsa::EcdsaOps},
        runtime::metrics::auth::{
            record_verifier_cert_expired, record_verifier_proof_mismatch,
            record_verifier_proof_missing,
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
            DelegatedTokenOpsError::CertSignatureInvalid(err.to_string()).into()
        }
        SignatureKind::Token => {
            DelegatedTokenOpsError::TokenSignatureInvalid(err.to_string()).into()
        }
    }
}

pub(super) fn verify_delegation_signature(proof: &DelegationProof) -> Result<(), InternalError> {
    if proof.cert_sig.is_empty() {
        return Err(DelegatedTokenOpsError::CertSignatureUnavailable.into());
    }

    let root_public_key = DelegationStateOps::root_public_key()
        .ok_or(DelegatedTokenOpsError::RootPublicKeyUnavailable)?;
    let hash = crypto::cert_hash(&proof.cert)?;
    EcdsaOps::verify_signature(&root_public_key, hash, &proof.cert_sig)
        .map_err(|err| map_signature_error(err, SignatureKind::Delegation))?;

    Ok(())
}

pub(super) fn verify_token_sig(token: &DelegatedToken) -> Result<(), InternalError> {
    if token.token_sig.is_empty() {
        return Err(DelegatedTokenOpsError::TokenSignatureUnavailable.into());
    }

    let shard_public_key = DelegationStateOps::shard_public_key(token.proof.cert.shard_pid).ok_or(
        DelegatedTokenOpsError::ShardPublicKeyUnavailable {
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
        return Err(DelegatedTokenOpsError::TokenExpiryBeforeIssued.into());
    }

    if now_secs < claims.iat {
        return Err(DelegatedTokenOpsError::TokenNotYetValid { iat: claims.iat }.into());
    }

    if now_secs > claims.exp {
        return Err(DelegatedTokenOpsError::TokenExpired { exp: claims.exp }.into());
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
        return Err(DelegatedTokenOpsError::CertExpired {
            expires_at: cert.expires_at,
        }
        .into());
    }

    if claims.iat < cert.issued_at {
        return Err(DelegatedTokenOpsError::TokenIssuedBeforeDelegation {
            token_iat: claims.iat,
            cert_iat: cert.issued_at,
        }
        .into());
    }

    if claims.exp > cert.expires_at {
        return Err(DelegatedTokenOpsError::TokenOutlivesDelegation {
            token_exp: claims.exp,
            cert_exp: cert.expires_at,
        }
        .into());
    }

    Ok(())
}

pub(super) fn verify_current_proof(proof: &DelegationProof) -> Result<(), InternalError> {
    let Some(stored) = DelegationStateOps::proof_dto() else {
        record_verifier_proof_missing();
        let local = IcOps::canister_self();
        crate::log!(
            crate::log::Topic::Auth,
            Warn,
            "delegation proof missing local={} shard={}",
            local,
            proof.cert.shard_pid
        );
        return Err(DelegatedTokenOpsError::ProofUnavailable.into());
    };

    if proofs_equal(proof, &stored) {
        Ok(())
    } else {
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
        Err(DelegatedTokenOpsError::ProofMismatch.into())
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
        return Err(DelegatedTokenOpsError::TokenTtlExceeded {
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
        Err(DelegatedTokenOpsError::SelfAudienceMissing { self_pid }.into())
    }
}

pub(super) fn validate_claims_against_cert(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
) -> Result<(), InternalError> {
    if claims.shard_pid != cert.shard_pid {
        return Err(DelegatedTokenOpsError::ShardPidMismatch {
            expected: cert.shard_pid,
            found: claims.shard_pid,
        }
        .into());
    }

    for aud in &claims.aud {
        if !cert.aud.iter().any(|allowed| allowed == aud) {
            return Err(DelegatedTokenOpsError::AudienceNotAllowed { aud: *aud }.into());
        }
    }

    for scope in &claims.scopes {
        if !cert.scopes.iter().any(|allowed| allowed == scope) {
            return Err(DelegatedTokenOpsError::ScopeNotAllowed {
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
        return Err(DelegatedTokenOpsError::AttestationSubjectMismatch {
            expected: caller,
            found: payload.subject,
        });
    }

    if now_secs > payload.expires_at {
        return Err(DelegatedTokenOpsError::AttestationExpired {
            expires_at: payload.expires_at,
            now_secs,
        });
    }

    if let Some(audience) = payload.audience
        && audience != self_pid
    {
        return Err(DelegatedTokenOpsError::AttestationAudienceMismatch {
            expected: self_pid,
            found: audience,
        });
    }

    if let Some(attestation_subnet) = payload.subnet_id {
        let verifier_subnet =
            verifier_subnet.ok_or(DelegatedTokenOpsError::AttestationSubnetUnavailable)?;
        if attestation_subnet != verifier_subnet {
            return Err(DelegatedTokenOpsError::AttestationSubnetMismatch {
                expected: verifier_subnet,
                found: attestation_subnet,
            });
        }
    }

    if payload.epoch < min_accepted_epoch {
        return Err(DelegatedTokenOpsError::AttestationEpochRejected {
            epoch: payload.epoch,
            min_accepted_epoch,
        });
    }

    Ok(())
}

pub(super) const fn verify_attestation_key_validity(
    key: &AttestationKey,
    now_secs: u64,
) -> Result<(), DelegatedTokenOpsError> {
    if let Some(valid_from) = key.valid_from
        && now_secs < valid_from
    {
        return Err(DelegatedTokenOpsError::AttestationKeyNotYetValid {
            key_id: key.key_id,
            valid_from,
            now_secs,
        });
    }

    if let Some(valid_until) = key.valid_until
        && now_secs > valid_until
    {
        return Err(DelegatedTokenOpsError::AttestationKeyExpired {
            key_id: key.key_id,
            valid_until,
            now_secs,
        });
    }

    Ok(())
}
