//!
//! ops::signature
//!
//! High-level wrapper around IC canister signatures.
//!
//! This allows a canister to sign arbitrary messages without holding any private key.
//!
//! Internally uses `ic_canister_sig_creation` and certified data to produce
//! verifiable, subnet-backed canister signatures.
//!
//! For verification, see: [`ic-standalone-sig-verifier`](https://crates.io/crates/ic-standalone-sig-verifier).
//!

use crate::{
    Error, ThisError, cdk::api::certified_data_set, ops::OpsError, serialize::deserialize,
    types::Principal,
};
use ic_canister_sig_creation::{
    CanisterSigPublicKey, IC_ROOT_PUBLIC_KEY, hash_with_domain, parse_canister_sig_cbor,
    signature_map::{CanisterSigInputs, LABEL_SIG, SignatureMap},
};
use ic_signature_verification::verify_canister_sig;
use serde::de::DeserializeOwned;
use std::cell::RefCell;

thread_local! {
    /// Transient signature map, kept in heap memory only.
    /// Entries expire automatically after ~1 minute.
    static SIGNATURES: RefCell<SignatureMap> = RefCell::new(SignatureMap::default());
}

const AUTH_SIGNATURE_DOMAIN: &[u8] = b"toko";
const AUTH_SIGNATURE_SEED: &[u8] = b"user-auth";

///
/// SignatureOpsError
///

#[derive(Debug, ThisError)]
pub enum SignatureOpsError {
    #[error("cannot parse signature")]
    CannotParseSignature,

    #[error("cannot parse tokens")]
    CannotParseTokens,

    #[error("invalid signature")]
    InvalidSignature,
}

impl From<SignatureOpsError> for Error {
    fn from(err: SignatureOpsError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// Prepares a canister signature for a given message and seed.
///
/// This updates the canister's `certified_data` to include the
/// new root hash so that the IC subnet will certify it.
///
/// - `seed` should uniquely identify the logical key context.
/// - `message` is the data being signed.
///
pub fn prepare(domain: &[u8], seed: &[u8], message: &[u8]) {
    let sig_inputs = CanisterSigInputs {
        domain,
        seed,
        message,
    };

    SIGNATURES.with_borrow_mut(|sigs| {
        sigs.add_signature(&sig_inputs);
    });

    // Commit new certified root
    SIGNATURES.with_borrow(|sigs| {
        certified_data_set(hash_with_domain(LABEL_SIG, &sigs.root_hash()));
    });
}

///
/// Retrieves a prepared canister signature as CBOR-encoded bytes.
///
/// Returns `None` if the signature has expired or was never prepared.
///
/// This is intended for use in query calls.
///
#[must_use]
pub fn get(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
    let sig_inputs = CanisterSigInputs {
        domain,
        seed,
        message,
    };

    SIGNATURES.with_borrow(|sigs| sigs.get_signature_as_cbor(&sig_inputs, None).ok())
}

///
/// High-level convenience helper that combines [`prepare`] and [`get`]
/// in one call. Suitable for simple use-cases where you don’t split update/query.
///
#[must_use]
pub fn sign(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
    prepare(domain, seed, message);
    get(domain, seed, message)
}

///
/// Verify a user token that was issued by the auth canister.
///
/// - `domain`:    the domain separator used during signing
/// - `seed`:      the seed that derived the signing public key
/// - `message`: the CBOR-encoded message Token
/// - `signature`:  the CBOR canister signature returned by auth
/// - `issuer_pid`: the Principal of the auth canister (the one that signed)
///
pub fn verify(
    domain: &[u8],
    seed: &[u8],
    message: &[u8],
    signature_cbor: &[u8],
    issuer_pid: Principal,
) -> Result<(), Error> {
    // 1️⃣ Parse CBOR
    parse_canister_sig_cbor(signature_cbor).map_err(|_| SignatureOpsError::CannotParseSignature)?;

    // 2️⃣ Verify the IC canister signature cryptographically
    let public_key = CanisterSigPublicKey::new(issuer_pid, seed.to_vec()).to_der();
    let domain_prefixed_message = domain_prefixed_message(domain, message);
    verify_canister_sig(
        &domain_prefixed_message,
        signature_cbor,
        &public_key,
        &IC_ROOT_PUBLIC_KEY,
    )
    .map_err(|_| SignatureOpsError::InvalidSignature)?;

    Ok(())
}

///
/// Verify a user token from the auth canister with the canonical domain/seed.
/// Keeps the domain/seed constants centralized to avoid call-site drift.
///
pub fn verify_auth_token(
    message: &[u8],
    signature_cbor: &[u8],
    issuer_pid: Principal,
) -> Result<(), Error> {
    verify(
        AUTH_SIGNATURE_DOMAIN,
        AUTH_SIGNATURE_SEED,
        message,
        signature_cbor,
        issuer_pid,
    )
}

///
/// Parses CBOR-encoded message bytes into a strongly-typed value `T`.
///
/// This is a thin convenience wrapper over [`deserialize`], ensuring that
/// all token deserialization uses the same canonical CBOR implementation.
///
pub fn parse_message<T>(message: &[u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let token = deserialize::<T>(message).map_err(|_| SignatureOpsError::CannotParseTokens)?;

    Ok(token)
}

///
/// Returns the canister’s current signature root hash.
/// Useful for debugging or introspection.
///
#[must_use]
pub fn root_hash() -> Vec<u8> {
    SIGNATURES.with_borrow(|sigs| sigs.root_hash().to_vec())
}

#[allow(clippy::cast_possible_truncation)]
fn domain_prefixed_message(domain: &[u8], message: &[u8]) -> Vec<u8> {
    // Mirror the preimage hashed by `hash_with_domain`.
    let mut buf = Vec::with_capacity(1 + domain.len() + message.len());
    buf.push(domain.len() as u8);
    buf.extend_from_slice(domain);
    buf.extend_from_slice(message);
    buf
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    use sha2::{Digest, Sha256};

    const TEST_SIGNING_CANISTER_ID: &str = "rwlgt-iiaaa-aaaaa-aaaaa-cai";
    const CANISTER_SIG_CBOR: &[u8; 265] = b"\xd9\xd9\xf7\xa2\x6b\x63\x65\x72\x74\x69\x66\x69\x63\x61\x74\x65\x58\xa1\xd9\xd9\xf7\xa2\x64\x74\x72\x65\x65\x83\x01\x83\x02\x48\x63\x61\x6e\x69\x73\x74\x65\x72\x83\x02\x4a\x00\x00\x00\x00\x00\x00\x00\x01\x01\x01\x83\x02\x4e\x63\x65\x72\x74\x69\x66\x69\x65\x64\x5f\x64\x61\x74\x61\x82\x03\x58\x20\xa9\xea\x05\x9d\xf2\x7a\x09\x7e\xc4\x38\xdb\x35\x62\xb9\x55\xc3\xd3\xfa\x08\xeb\x17\xc1\x3c\xda\x63\x90\x42\xfa\xe0\xcf\x60\x36\x83\x02\x44\x74\x69\x6d\x65\x82\x03\x43\x87\xad\x4b\x69\x73\x69\x67\x6e\x61\x74\x75\x72\x65\x58\x30\xa4\xd5\xfd\x47\xa0\x88\x13\x5b\xed\x52\x22\x0c\xca\xa4\x76\xfb\x6c\x88\x95\xdd\xa3\x1e\x2a\x86\xa7\xa2\x97\xdc\x7a\x30\x81\x27\x1e\xf1\x1a\xee\xb5\xd2\xbb\x25\x83\x0d\xcb\xdd\x82\xad\x7a\x52\x64\x74\x72\x65\x65\x83\x02\x43\x73\x69\x67\x83\x02\x58\x20\x00\x42\xcd\x04\x7a\xad\x32\x06\x37\xce\xae\xe2\x1d\x48\x9e\xf4\xe5\x14\xce\x20\x1f\x19\x60\x68\x30\xa2\xaf\x7b\x7d\x9c\x86\x7d\x83\x02\x58\x20\x14\x9b\x80\x95\x11\x98\x27\xcf\xea\x0a\xa6\x6e\x7b\x7f\x80\xe9\x13\xca\xef\xa3\x1a\x60\x6d\xe4\x02\x69\xc3\xd8\x6c\xfe\xa5\x8d\x82\x03\x40";

    #[test]
    fn domain_prefix_matches_hash_with_domain() {
        let domain = b"domain";
        let message = b"payload";

        let preimage = domain_prefixed_message(domain, message);

        let mut hasher = Sha256::new();
        hasher.update(&preimage);
        let digest: [u8; 32] = hasher.finalize().into();

        assert_eq!(digest, hash_with_domain(domain, message));
    }

    #[test]
    fn verify_auth_token_handles_short_principal_without_panicking() {
        let issuer_pid = Principal::from_text(TEST_SIGNING_CANISTER_ID).unwrap();
        let err = verify_auth_token(b"payload", CANISTER_SIG_CBOR, issuer_pid)
            .expect_err("expected invalid signature, not success");
        assert_eq!(err.to_string(), "invalid signature");
    }
}
