use super::ROLE_ATTESTATION_SIGNING_DOMAIN;
use crate::{
    InternalError,
    dto::auth::RoleAttestation,
    ops::{auth::AuthValidationError, prelude::*},
};
use candid::encode_one;
use sha2::{Digest, Sha256};

pub(super) fn encode_candid<T: CandidType>(
    context: &'static str,
    value: &T,
) -> Result<Vec<u8>, InternalError> {
    encode_one(value).map_err(|err| {
        AuthValidationError::EncodeFailed {
            context,
            source: err,
        }
        .into()
    })
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
