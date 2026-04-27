use super::{
    CERT_SIGNING_DOMAIN, ROLE_ATTESTATION_SIGNING_DOMAIN, TOKEN_SIGNING_DOMAIN, VerifiedTokenClaims,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{DelegationAudience, DelegationCert, RoleAttestation},
    ops::{auth::DelegationValidationError, prelude::*},
};
use candid::encode_one;
use sha2::{Digest, Sha256};

//
// TokenSigningPayload
//

#[derive(CandidType)]
struct TokenSigningPayload {
    cert_hash: Vec<u8>,
    claims: VerifiedTokenClaims,
}

pub(super) fn encode_candid<T: CandidType>(
    context: &'static str,
    value: &T,
) -> Result<Vec<u8>, InternalError> {
    encode_one(value).map_err(|err| {
        DelegationValidationError::EncodeFailed {
            context,
            source: err,
        }
        .into()
    })
}

pub(super) fn cert_hash(cert: &DelegationCert) -> [u8; 32] {
    hash_delegation_cert(cert)
}

pub(super) fn token_signing_hash(
    claims: &VerifiedTokenClaims,
    cert: &DelegationCert,
) -> Result<[u8; 32], InternalError> {
    let payload = TokenSigningPayload {
        cert_hash: cert_hash(cert).to_vec(),
        claims: claims.clone(),
    };

    let encoded = encode_candid("token signing payload", &payload)?;
    Ok(hash_domain_separated(TOKEN_SIGNING_DOMAIN, &encoded))
}

pub(super) fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    hasher.finalize().into()
}

fn hash_delegation_cert(cert: &DelegationCert) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((CERT_SIGNING_DOMAIN.len() as u64).to_be_bytes());
    hasher.update(CERT_SIGNING_DOMAIN);
    update_principal(&mut hasher, cert.root_pid);
    update_principal(&mut hasher, cert.shard_pid);
    hasher.update(cert.issued_at.to_be_bytes());
    hasher.update(cert.expires_at.to_be_bytes());
    update_strings(&mut hasher, &cert.scopes);
    update_audience(&mut hasher, &cert.aud);
    hasher.finalize().into()
}

fn update_principal(hasher: &mut Sha256, principal: Principal) {
    update_bytes(hasher, principal.as_slice());
}

fn update_audience(hasher: &mut Sha256, audience: &DelegationAudience) {
    match audience {
        DelegationAudience::Any => {
            hasher.update(0u8.to_be_bytes());
        }
        DelegationAudience::Roles(roles) => {
            hasher.update(1u8.to_be_bytes());
            hasher.update((roles.len() as u64).to_be_bytes());
            for role in roles {
                update_bytes(hasher, role.as_str().as_bytes());
            }
        }
    }
}

fn update_strings(hasher: &mut Sha256, values: &[String]) {
    hasher.update((values.len() as u64).to_be_bytes());
    for value in values {
        update_bytes(hasher, value.as_bytes());
    }
}

fn update_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

pub(super) fn role_attestation_hash(
    attestation: &RoleAttestation,
) -> Result<[u8; 32], InternalError> {
    let payload = encode_candid("role attestation", attestation)?;
    let mut hasher = Sha256::new();
    hasher.update(ROLE_ATTESTATION_SIGNING_DOMAIN);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}
