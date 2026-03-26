use super::{
    CERT_SIGNING_DOMAIN, ROLE_ATTESTATION_SIGNING_DOMAIN, TOKEN_SIGNING_DOMAIN, VerifiedTokenClaims,
};
use crate::{
    InternalError,
    dto::auth::{DelegationCert, RoleAttestation},
    ops::{auth::DelegationValidationError, prelude::*},
};
use candid::encode_one;
use sha2::{Digest, Sha256};

///
/// TokenSigningPayload
///

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

pub(super) fn cert_hash(cert: &DelegationCert) -> Result<[u8; 32], InternalError> {
    let payload = encode_candid("delegation cert", cert)?;
    Ok(hash_domain_separated(CERT_SIGNING_DOMAIN, &payload))
}

pub(super) fn token_signing_hash(
    claims: &VerifiedTokenClaims,
    cert: &DelegationCert,
) -> Result<[u8; 32], InternalError> {
    let payload = TokenSigningPayload {
        cert_hash: cert_hash(cert)?.to_vec(),
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

pub(super) fn role_attestation_hash(
    attestation: &RoleAttestation,
) -> Result<[u8; 32], InternalError> {
    let payload = encode_candid("role attestation", attestation)?;
    let mut hasher = Sha256::new();
    hasher.update(ROLE_ATTESTATION_SIGNING_DOMAIN);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}
