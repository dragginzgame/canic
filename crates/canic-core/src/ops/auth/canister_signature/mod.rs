//! Module: ops::auth::canister_signature
//!
//! Responsibility: bind protected signer/seed context and verify one canister signature.
//! Does not own: certificate freshness policy, signature creation or authorization.
//! Boundary: IC Auth owns DER admission, cryptography, framing and certificate freshness.

use crate::{
    cdk::types::Principal, dto::auth::IcCanisterSignatureProofV1, ops::auth::AuthSignatureError,
};
use sha2::{Digest, Sha256};

///
/// CanisterSignatureVerificationInput
///
/// Product-selected signature context, independent of the submitted public key.
///

pub(super) struct CanisterSignatureVerificationInput<'a> {
    pub payload_hash: [u8; 32],
    pub proof: &'a IcCanisterSignatureProofV1,
    pub signing_canister: Principal,
    pub seed: &'a [u8],
    pub domain: &'a [u8],
    pub ic_root_public_key_raw: &'a [u8],
    pub now_ns: u64,
    pub max_certificate_age_ns: u64,
}

pub(super) fn verify_canister_signature(
    input: CanisterSignatureVerificationInput<'_>,
) -> Result<(), AuthSignatureError> {
    let message =
        ic_auth::canister_signature::domain_separated_message(input.domain, input.payload_hash)
            .map_err(|error| AuthSignatureError::ProofInvalid(error.to_string()))?;
    ic_auth::canister_signature::verify_canister_signature(
        &message,
        &ic_auth_protocol_types::IcCanisterSignatureProofV1 {
            signature_cbor: input.proof.signature_cbor.clone(),
            public_key_der: input.proof.public_key_der.clone(),
        },
        &ic_auth::canister_signature::CanisterSignaturePolicy {
            signing_canister: input.signing_canister.into(),
            seed_hash: Sha256::digest(input.seed).into(),
            ic_root_public_key_raw: input.ic_root_public_key_raw,
            now_ns: input.now_ns,
            max_certificate_age_ns: input.max_certificate_age_ns,
            max_future_skew_ns: super::AUTH_TIME_SKEW_ALLOWANCE_NS,
            max_signature_bytes: 256 * 1024,
            max_message_bytes: 1 + 255 + 32,
        },
    )
    .map_err(|error| AuthSignatureError::ProofInvalid(error.to_string()))
}
